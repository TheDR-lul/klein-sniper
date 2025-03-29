use crate::model::Offer;
use crate::config::ModelConfig;

pub fn normalize_all(offers: &mut Vec<Offer>, models: &[ModelConfig]) {
    for offer in offers.iter_mut() {
        normalize_offer(offer, models);
    }
}

fn normalize_offer(offer: &mut Offer, models: &[ModelConfig]) {
    let title = offer.title.to_lowercase();

    for model in models {
        for keyword in &model.match_keywords {
            if title.contains(&keyword.to_lowercase()) {
                offer.model = model.query.clone(); // ✅ фикс: теперь присваивается основное имя из конфига
                return;
            }
        }
    }

    offer.model = "unknown".to_string();
}